import { Component, Input } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

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
    const formattedTitle = this.formatKey(newTitle);
    const resolvedTitle = this.resolveTitle(formattedTitle);
    this.currentTitle = resolvedTitle ? resolvedTitle : newTitle;
    this.titleService.setTitle(this.currentTitle);
    this.updatePath();
  };
  @Input() set page(newPage: string) {
    const formattedPage = this.formatKey(newPage);
    let resolvedPage = this.resolvePage(formattedPage);
    if (!newPage && this.currentTitle) {
      resolvedPage = 'page_01'; // Valor por defecto
      this.router.navigate(['/test', this.validLang, this.currentTitle, resolvedPage]);
    } 
    if (newPage && this.currentTitle) {
      this.currentPage = resolvedPage ? resolvedPage : formattedPage;
      this.pageService.setCurrentPage(this.currentPage);
      this.updatePath();
    }
    this.currentPage = resolvedPage ? resolvedPage : formattedPage;
    this.pageService.setCurrentPage(this.currentPage);
    this.updatePath();
  }

  constructor(
    private router: Router,
    private activatedRoute: ActivatedRoute,
    private language: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
  ) {
    this.activatedRoute.params.subscribe(params => {
      if (!params['page']) {
        const lang = params['lang'];
        const title = params['title'];
        const defaultPage = 'into';
        this.router.navigate(['/test', lang, title, defaultPage]);
      }
    });
  }

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

  formatKey(input: string): string {
    return input.toLowerCase().replace(/[-_]/g, '');
  }

  private updatePath(): void {
    this.path = null; // When the path is updated, it forces a re-render
    setTimeout(() => {
      this.path = [this.validLang, this.currentTitle, this.currentPage];
    }, 0);
  }

}