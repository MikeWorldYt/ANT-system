import { CommonModule } from '@angular/common';
import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';
import { PageService } from '../../../services/navPage.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'doc-t02-mod-03-nomenclature',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './mod-03-nomenclature.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod03_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: ArticleService,
    private languageService: LanguageService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'ES';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';
  
  // ▬▬▬ Hash Sections
  hovered: boolean = false;
  
  private languageSubscription: Subscription = new Subscription();
  
  // ▲ Hash Sections Service
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ███ OnInit ███ 
  ngOnInit(): void {
    this.initializeContent();
    this.subscribeToLanguageChanges();
    this.pageService.setCurrentPage('nomenclature');
  }

  private initializeContent(): void {
    this.write = content[this.currentLanguage].title_02.page_03;
  }

  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.page_03;
        this.currentLanguage = language;
      }
    });
  }

  // ███ Language Controller ███
  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  // ███ Hash Sections ███
  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ████ AfterViewInit
  ngAfterViewInit() {
    // Intersection Section
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
  }

}
