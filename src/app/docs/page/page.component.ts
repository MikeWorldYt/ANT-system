import { Component, Input } from '@angular/core';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { TitleService } from '../../services/navTitle.service';
import { PageService } from '../../services/navPage.service';

// ▲ RESOLVE  ▲
import { pathResolve } from '../content/pathResolve';

// ▲ FEATURES ▲
import { ContentPageComponent } from '../content-page/content-page.component';

@Component({
  selector: 'app-page',
  standalone: true,
  imports: [
    ContentPageComponent,
  ],
  templateUrl: './page.component.html',
  styleUrl: './page.component.css'
})
export class DocsPageComponent {
  @Input() set lang(newLang: string){
    this.validLang = (newLang === 'EN' || newLang === 'ES') ? newLang : 'EN';
    this.language.setLanguage(this.validLang);
    this.updatePath();
  };
  @Input() set title (newTitle: string) {
    const resolvedTitle = this.resolveTitle(newTitle);
    this.currentTitle = resolvedTitle ? resolvedTitle : newTitle;
    this.titleService.setTitle(this.currentTitle);
    this.updatePath();
  };
  @Input() set page(newPage: string) {
    const resolvedPage = this.resolvePage(newPage);
    this.currentPage = resolvedPage ? resolvedPage : newPage;
    this.pageService.setCurrentPage(this.currentPage);
    this.updatePath();
  }

  constructor(
    private language: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ Navigation Context
  validLang: string = '';
  currentTitle: string = '';
  currentPage: string = '';
  path: [string, string, string] | null = null;

  resolveTitle(titleKey: string): string | null {
    if (pathResolve[titleKey] && pathResolve[titleKey].key) {
      return pathResolve[titleKey].key;
    }
    return null;
  }

  resolvePage(pageKey: string): string | null {
    const section = Object.keys(pathResolve).find(
      key => pathResolve[key].key === this.currentTitle
    );
    if (section && pathResolve[section][pageKey]) {
      return pathResolve[section][pageKey];
    }
    return null;
  }

  private updatePath(): void {
    this.path = null; // When the path is updated, it forces a re-render
    setTimeout(() => {
      this.path = [this.validLang, this.currentTitle, this.currentPage];
    }, 0);
  }

}